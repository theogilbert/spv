//! Time measure and associated tools

/// Represents an iteration of the program's main loop
pub type Iteration = usize;

/// Keeps track of the current iteration of the program
pub struct IterationTracker {
    counter: usize,
}

impl Default for IterationTracker {
    fn default() -> Self {
        IterationTracker { counter: 0 }
    }
}

impl IterationTracker {
    pub fn tick(&mut self) {
        self.counter += 1
    }

    pub fn current(&self) -> usize {
        self.counter
    }
}

#[cfg(test)]
mod test_iteration {
    use rstest::*;

    use crate::core::iteration::IterationTracker;

    #[fixture]
    fn iteration_tracker() -> IterationTracker {
        IterationTracker::default()
    }

    #[rstest]
    fn test_iteration_should_be_0_by_default(iteration_tracker: IterationTracker) {
        assert_eq!(iteration_tracker.current(), 0);
    }

    #[rstest]
    #[case(1)]
    #[case(5)]
    fn test_iteration_should_increase_on_tick(mut iteration_tracker: IterationTracker, #[case] tick_count: usize) {
        for _ in 0..tick_count {
            iteration_tracker.tick();
        }

        assert_eq!(iteration_tracker.current(), tick_count);
    }
}

/// Represents a temporal region, expressed using iterations
///
/// A `Span` has a `begin`, an `end` and a `size`.
/// The `begin` and `end` are inclusive.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Span {
    begin: Iteration,
    end: Iteration,
    size: Iteration, // size is required as an attribute, to represent spans including the iteration 0 (e.g. [-59;0])
}

impl Span {
    #[cfg(test)]
    pub fn new(begin: Iteration, end: Iteration) -> Self {
        Span {
            begin,
            end,
            size: end - begin + 1,
        }
    }

    /// Creates a `Span` starting at the given `Iteration`
    /// The `Span` will have a size of 1. This means that it ends at the same `Iteration` than `begin`.
    ///
    /// # Arguments
    /// * `begin`: The left-bound iteration of the span
    pub fn from_begin(begin: Iteration) -> Self {
        Span {
            begin,
            end: begin,
            size: 1,
        }
    }

    /// Creates a `Span` with the given size.
    /// The `Span` ends at the `Iteration` 0.
    /// To update the end of the span, see [`set_end_and_shift`](#method.set_end_and_shift)
    ///
    /// A span must have a span of at least 1 iteration. Attempting to create a span with a size of 0 will panic.
    ///
    /// # Arguments
    /// * `size`: The size of the `Span`. It must at least 1.
    pub fn from_size(size: Iteration) -> Self {
        if size == 0 {
            panic!("Invalid size for span: 0")
        }
        Span { begin: 0, end: 0, size }
    }

    /// Updates the end of the span and updates the `begin` attribute using the `size` attribute.
    /// After this operation, the size of the span will remain the same.
    ///
    /// # Arguments
    /// * `end`: The last iteration covered by the span
    pub fn set_end_and_shift(&mut self, end: Iteration) {
        self.end = end;
        self.begin = end.checked_sub(self.size).map(|v| v + 1).unwrap_or(Iteration::MIN);
    }

    /// Updates the end of the span and updates the `size` attribute using the `begin` attribute
    /// After this operation, the `begin` iteration of the span will remain the same.
    ///
    /// This method panics if `end` is less than `begin`.
    ///
    /// # Arguments
    /// * `end`: The last iteration covered by the span
    pub fn set_end_and_resize(&mut self, end: Iteration) {
        self.end = end;
        self.size = self.end - self.begin + 1;
    }

    /// Updates the span by offseting the `begin` and `end` attributes of the span
    ///
    /// The span cannot be scrolled before the iteration 0 or after the current iteration.
    /// This means that `begin` can never be less than 0, and `end` can never be greater than the current iteration.
    ///
    /// # Arguments
    /// * `current_iteration`: Indicates the rightmost limit of the span
    /// * `delta`: Indicates by how many iterations to shift the span.<br/>
    ///             A negative number shifts the span toward the iteration 0.<br/>
    ///             A positive number shifts the span toward the current iteration. <br/>
    pub fn scroll(&mut self, current_iteration: Iteration, delta: i64) {
        let projected_end = (self.end as i64 + delta).max(0) as Iteration;
        let bounded_end = projected_end.max(self.size - 1).min(current_iteration);
        self.set_end_and_shift(bounded_end);
    }

    /// Indicates if the span is fully scrolled to the right (toward the current iteration) or if it can be further
    /// scrolled to the right.
    ///
    /// # Arguments
    /// * `current_iteration` The current iteration of the program
    pub fn is_fully_scrolled_right(&self, current_iteration: Iteration) -> bool {
        current_iteration == self.end
    }

    /// Returns the first iteration covered by the span.
    /// This value can never be greater than `self.end()`
    pub fn begin(&self) -> Iteration {
        self.begin
    }

    /// Returns the first iteration covered by the span, even if this iteration is negative.
    ///
    /// It is possible for a `Span` to start at a negative iteration, if `size` is greater than `end`.
    pub fn signed_begin(&self) -> i128 {
        self.end as i128 - self.size as i128 + 1
    }

    /// Returns the last iteration covered by the span
    /// This value can never be less than `self.begin()`
    pub fn end(&self) -> Iteration {
        self.end
    }

    /// Returns the amount of iterations covered by the span.<br/>
    /// Note that the `size` does not necessarily represent the difference between the `begin` and `end` of the span.
    /// That can be the case when we want to represent a span with a fixed `size` greater than the `end` of the span.
    /// As the first iteration of the span cannot have a negative value (represented as a `usize`), `size` will be
    /// greater than the difference between `begin` and `end`.
    pub fn size(&self) -> Iteration {
        self.size
    }

    /// Returns true if `self` intersects with `other`
    ///
    /// # Arguments
    /// * `other`: A `Span` reference for which to test an intersection with `self`
    pub fn intersects(&self, other: &Span) -> bool {
        !(self.end < other.begin || self.begin > other.end)
    }
}

#[cfg(test)]
mod test_span {
    use rstest::*;

    use crate::core::iteration::{Iteration, Span};

    #[test]
    fn test_should_correctly_define_span_when_creating_from_begin() {
        let span = Span::from_begin(60);

        assert_eq!(span.begin(), 60);
        assert_eq!(span.end(), 60);
        assert_eq!(span.size(), 1);
    }

    #[test]
    fn test_should_correctly_define_span_when_creating_from_size() {
        let span = Span::from_size(60);

        assert_eq!(span.begin(), 0);
        assert_eq!(span.end(), 0);
        assert_eq!(span.size(), 60);
    }

    #[test]
    fn test_should_update_begin_when_setting_end_and_updating_begin() {
        let mut span = Span::from_size(60);
        span.set_end_and_shift(180);

        assert_eq!(span.begin(), 121);
    }

    #[test]
    fn test_should_prevent_underflow_when_setting_end_and_updating_begin() {
        let mut span = Span::from_size(60);
        span.set_end_and_shift(30);

        assert_eq!(span.begin(), 0);
    }

    #[test]
    fn test_should_update_end_when_setting_end_and_updating_begin() {
        let mut span = Span::from_size(60);
        span.set_end_and_shift(180);

        assert_eq!(span.end(), 180);
    }

    #[test]
    fn test_should_not_update_size_when_setting_end_and_updating_begin() {
        let mut span = Span::from_size(60);
        span.set_end_and_shift(180);

        assert_eq!(span.size(), 60);
    }

    #[test]
    fn test_should_update_size_when_setting_end_and_updating_size() {
        let mut span = Span::from_begin(121);
        span.set_end_and_resize(240);

        assert_eq!(span.size(), 120);
    }

    #[test]
    fn test_should_update_end_when_setting_end_and_updating_size() {
        let mut span = Span::from_begin(121);
        span.set_end_and_resize(180);

        assert_eq!(span.end(), 180);
    }

    #[test]
    fn test_should_not_update_begin_when_setting_end_and_updating_size() {
        let mut span = Span::from_begin(121);
        span.set_end_and_resize(180);

        assert_eq!(span.begin(), 121);
    }

    #[rstest]
    #[case(50, 250)]
    #[case(50, 100)]
    #[case(120, 170)]
    #[case(199, 250)]
    fn test_should_return_true_if_spans_intersect(#[case] begin_other: Iteration, #[case] end_other: Iteration) {
        let span = Span::new(100, 199);
        let other_span = Span::new(begin_other, end_other);

        assert!(span.intersects(&other_span));
    }

    #[rstest]
    #[case(50, 75)]
    #[case(250, 275)]
    fn test_should_return_false_if_spans_do_not_intersect(
        #[case] begin_other: Iteration,
        #[case] end_other: Iteration,
    ) {
        let span = Span::new(100, 199);
        let other_span = Span::new(begin_other, end_other);

        assert!(!span.intersects(&other_span));
    }

    #[test]
    fn test_should_return_negative_begin_when_size_greater_than_end() {
        let mut span = Span::from_size(60);
        span.set_end_and_shift(30);

        assert_eq!(span.signed_begin(), -29);
    }

    #[test]
    fn test_should_return_positive_begin_when_size_less_than_end() {
        let mut span = Span::from_size(60);
        span.set_end_and_shift(120);

        assert_eq!(span.signed_begin(), 61);
    }

    #[test]
    fn test_should_scroll_to_the_right() {
        let mut span = Span::new(10, 20);
        span.scroll(100, 10);

        assert_eq!(span, Span::new(20, 30));
    }

    #[test]
    fn test_should_scroll_to_the_left() {
        let mut span = Span::new(20, 30);
        span.scroll(100, -10);

        assert_eq!(span, Span::new(10, 20));
    }

    #[test]
    fn test_should_not_scroll_before_iteration_0() {
        let mut span = Span::new(20, 30);
        span.scroll(100, -50);

        assert_eq!(span, Span::new(0, 10));
    }

    #[test]
    fn test_should_not_scroll_after_current_iteration() {
        let mut span = Span::new(20, 30);
        span.scroll(100, 100);

        assert_eq!(span, Span::new(90, 100));
    }

    #[test]
    fn test_should_be_fully_scrolled_to_the_right_by_default() {
        let span = Span::from_size(60);

        assert!(span.is_fully_scrolled_right(0));
    }

    #[test]
    fn test_should_be_fully_scrolled_to_the_right_when_shifted_to_current_iteration() {
        let mut span = Span::from_size(60);
        span.set_end_and_shift(1);

        assert!(span.is_fully_scrolled_right(1));
    }

    #[test]
    fn test_should_not_be_fully_scrolled_to_the_right_when_not_ends_at_current_iteration() {
        let span = Span::from_size(60);

        assert!(!span.is_fully_scrolled_right(1));
    }
}

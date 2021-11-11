/// On every iteration of the application's main loop, all processes will be probed for their metrics
/// An Iteration value refers to one of these iterations
pub type Iteration = usize;

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
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Span {
    begin: Iteration,
    end: Iteration,
    size: Iteration, // iteration is required as an attribute, for the cases where size > end (as begin cannot be < 0)
}

impl Span {
    #[cfg(test)]
    pub fn from_end_and_size(end: Iteration, size: Iteration) -> Self {
        Span {
            begin: end
                .checked_sub(size)
                .and_then(|v| Some(v + 1))
                .unwrap_or(Iteration::MIN),
            end,
            size,
        }
    }
    pub fn from_begin(begin: Iteration) -> Self {
        Span {
            begin,
            end: begin,
            size: 1,
        }
    }

    pub fn from_size(size: Iteration) -> Self {
        Span { begin: 0, end: 0, size }
    }

    /// Updates the end of the span and updates the `begin` attribute using the `size` attribute
    /// # Arguments
    /// * `end`: The last iteration covered by the span
    pub fn set_end_and_update_begin(&mut self, end: Iteration) {
        self.end = end;
        self.begin = end
            .checked_sub(self.size)
            .and_then(|v| Some(v + 1))
            .unwrap_or(Iteration::MIN);
    }

    /// Updates the end of the span and updates the `size` attribute using the `begin` attribute
    ///
    /// This method panics if `end` is less than `begin`.
    ///
    /// # Arguments
    /// * `end`: The last iteration covered by the span
    pub fn set_end_and_update_size(&mut self, end: Iteration) {
        self.end = end;
        self.size = self.end.checked_sub(self.begin).unwrap() + 1;
    }

    /// Returns the first iteration covered by the span
    /// This value can never be greater than `self.end()`
    pub fn begin(&self) -> Iteration {
        self.begin
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
        span.set_end_and_update_begin(180);

        assert_eq!(span.begin(), 121);
    }

    #[test]
    fn test_should_prevent_underflow_when_setting_end_and_updating_begin() {
        let mut span = Span::from_size(60);
        span.set_end_and_update_begin(30);

        assert_eq!(span.begin(), 0);
    }

    #[test]
    fn test_should_update_end_when_setting_end_and_updating_begin() {
        let mut span = Span::from_size(60);
        span.set_end_and_update_begin(180);

        assert_eq!(span.end(), 180);
    }

    #[test]
    fn test_should_not_update_size_when_setting_end_and_updating_begin() {
        let mut span = Span::from_size(60);
        span.set_end_and_update_begin(180);

        assert_eq!(span.size(), 60);
    }

    #[test]
    fn test_should_update_size_when_setting_end_and_updating_size() {
        let mut span = Span::from_begin(121);
        span.set_end_and_update_size(240);

        assert_eq!(span.size(), 120);
    }

    #[test]
    fn test_should_update_end_when_setting_end_and_updating_size() {
        let mut span = Span::from_begin(121);
        span.set_end_and_update_size(180);

        assert_eq!(span.end(), 180);
    }

    #[test]
    fn test_should_not_update_begin_when_setting_end_and_updating_size() {
        let mut span = Span::from_begin(121);
        span.set_end_and_update_size(180);

        assert_eq!(span.begin(), 121);
    }

    #[rstest]
    #[case(50, 250)]
    #[case(50, 101)]
    #[case(120, 170)]
    #[case(200, 250)]
    fn test_should_return_true_if_spans_intersect(#[case] begin_other: Iteration, #[case] end_other: Iteration) {
        let span = Span::from_end_and_size(200, 100);
        let other_span = Span::from_end_and_size(end_other, end_other - begin_other + 1);

        assert!(span.intersects(&other_span));
    }

    #[rstest]
    #[case(50, 75)]
    #[case(250, 275)]
    fn test_should_return_false_if_spans_do_not_intersect(
        #[case] begin_other: Iteration,
        #[case] end_other: Iteration,
    ) {
        let span = Span::from_end_and_size(200, 100);
        let other_span = Span::from_end_and_size(end_other, end_other - begin_other + 1);

        assert!(!span.intersects(&other_span));
    }
}

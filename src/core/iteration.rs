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
#[derive(Copy, Clone)]
pub struct Span {
    size: Iteration,
    end: Iteration,
}

impl Default for Span {
    fn default() -> Self {
        const DEFAULT_SPAN_WIDTH: Iteration = 60;
        Span {
            end: 0,
            size: DEFAULT_SPAN_WIDTH,
        }
    }
}

impl Span {
    #[cfg(test)]
    pub fn new(span: Iteration, end: Iteration) -> Self {
        Span { size: span, end }
    }

    pub fn set_end(&mut self, iteration: Iteration) {
        self.end = iteration;
    }

    pub fn size(&self) -> Iteration {
        self.size
    }

    pub fn begin(&self) -> Iteration {
        self.end.checked_sub(self.size).unwrap_or(Iteration::MIN)
    }

    pub fn end(&self) -> Iteration {
        self.end
    }

    // TODO implement intersect(span) and contains(iteration) if needed
}

#[cfg(test)]
mod test_iteration_span {
    use crate::core::iteration::Span;

    #[test]
    fn test_should_update_begin_when_setting_end() {
        let mut span = Span::default();
        span.set_end(180);

        assert_eq!(span.begin(), 180 - span.size());
    }

    #[test]
    fn test_should_update_end_when_setting_end() {
        let mut span = Span::default();
        span.set_end(123);

        assert_eq!(span.end(), 123);
    }
}

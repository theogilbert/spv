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

#[derive(Copy, Clone)]
pub struct IterSpan {
    span: usize,
}

impl Default for IterSpan {
    fn default() -> Self {
        IterSpan {
            span: 60, // Default hard-coded value, at 1 iteration/s, is a span of 1 minute
        }
    }
}

impl IterSpan {
    #[cfg(test)]
    pub fn new(span: usize) -> Self {
        IterSpan { span }
    }

    pub fn begin(&self, current_iteration: Iteration) -> Iteration {
        current_iteration.checked_sub(self.span).unwrap_or(Iteration::MIN)
    }

    pub fn span(&self) -> usize {
        self.span
    }
}

#[cfg(test)]
mod test_iter_span {
    use crate::core::iteration::IterSpan;

    #[test]
    fn test_should_substract_60_iteration_to_get_begin() {
        let span = IterSpan::default();
        assert_eq!(span.begin(120), 60);
    }
}

//! Manages the configuration of the span of metrics to render
use std::time::Duration;

use crate::core::time::{Span, Timestamp};

pub struct RenderingSpan {
    span: Span,
    tolerance: Duration,
    follow: bool,
}

impl RenderingSpan {
    /// # Arguments
    /// - `duration`: Indicates the amount of time that the span covers
    /// - `tolerance`: Tracking time precisely to the nanosecond is difficult.<br/>
    ///     The tolerance, will loosen the constraints of the span, by shifting its begin to the past.
    pub fn new(duration: Duration, tolerance: Duration) -> Self {
        Self {
            span: Span::from_duration(duration),
            tolerance,
            follow: true,
        }
    }

    /// Refreshes the rendering span so that it keeps being scrolled to the right
    pub fn refresh(&mut self) {
        // TODO find a more appropriate name for the method
        if self.follow {
            self.span.set_end_and_shift(Timestamp::now());
            self.set_follow_if_span_is_tracking_current_timestamp();
        }
    }
    /// Updates the span by offseting the `begin` and `end` attributes of the span toward the past
    ///
    /// The span cannot be scrolled before the first iteration of the program
    pub fn scroll_left(&mut self) {
        self.set_bounded_end_and_shift(self.span.end() - Duration::from_secs(1));
        self.set_follow_if_span_is_tracking_current_timestamp();
    }

    /// Updates the span by offseting the `begin` and `end` attributes of the span toward the future
    ///
    /// The span cannot be scrolled after the current timestamp.
    pub fn scroll_right(&mut self) {
        self.set_bounded_end_and_shift(self.span.end() + Duration::from_secs(1));
        self.set_follow_if_span_is_tracking_current_timestamp();
    }

    /// Reset the span so that it tracks the latest metrics
    pub fn reset_scroll(&mut self) {
        self.span.set_end_and_shift(Timestamp::now());
        self.set_follow_if_span_is_tracking_current_timestamp();
    }

    fn set_follow_if_span_is_tracking_current_timestamp(&mut self) {
        self.follow = self.span.end() == Timestamp::now();
    }

    /// Returns the actual `Span` representing the scope to render
    pub fn to_span(&self) -> Span {
        Span::new(self.span.begin() - self.tolerance, self.span.end())
    }

    #[cfg(test)]
    pub fn tolerance(&self) -> Duration {
        self.tolerance
    }

    /// Sets the end of the span and shift it (without resizing it)
    /// The end is capped so that the span cannot cover a time before the application started, or after the current time
    fn set_bounded_end_and_shift(&mut self, unbounded_end: Timestamp) {
        let min_end = Timestamp::app_init() + self.span.duration();
        let max_end = Timestamp::now();
        let bounded_end = unbounded_end.max(min_end).min(max_end);
        self.span.set_end_and_shift(bounded_end);
    }
}

#[cfg(test)]
mod test_rendering_span {
    use std::time::Duration;

    use rstest::*;

    use crate::core::time::test_utils::{
        advance_time_and_refresh_timestamp, setup_fake_clock_to_prevent_substract_overflow,
    };
    use crate::core::time::{Span, Timestamp};
    use crate::ctrl::span::RenderingSpan;

    #[fixture]
    fn rendering_span() -> RenderingSpan {
        setup_fake_clock_to_prevent_substract_overflow();
        RenderingSpan::new(Duration::from_secs(60), Duration::from_secs(1))
    }

    #[rstest]
    fn test_should_end_at_current_timestamp_by_default(rendering_span: RenderingSpan) {
        assert_eq!(rendering_span.to_span().end(), Timestamp::now());
    }

    #[rstest]
    fn test_should_scroll_to_the_right(mut rendering_span: RenderingSpan) {
        let original_span = rendering_span.to_span();

        advance_time_and_refresh_timestamp(Duration::from_secs(10));
        rendering_span.scroll_right();

        let new_span = rendering_span.to_span();

        assert!(new_span.begin() > original_span.begin());
        assert!(new_span.end() > original_span.end());
        assert_eq!(original_span.duration(), new_span.duration());
    }

    #[rstest]
    fn test_should_not_scroll_past_the_current_timestamp(mut rendering_span: RenderingSpan) {
        let original_span = rendering_span.to_span();

        rendering_span.scroll_right();

        let new_span = rendering_span.to_span();

        assert_eq!(original_span.begin(), new_span.begin());
        assert_eq!(original_span.end(), new_span.end());
        assert_eq!(original_span.duration(), new_span.duration());
    }

    #[rstest]
    fn test_should_scroll_to_the_left(mut rendering_span: RenderingSpan) {
        let original_span = rendering_span.to_span();

        advance_time_and_refresh_timestamp(Duration::from_secs(10));
        rendering_span.scroll_left();

        let new_span = rendering_span.to_span();

        assert!(new_span.begin() < original_span.begin());
        assert!(new_span.end() < original_span.end());
        assert_eq!(original_span.duration(), new_span.duration());
    }

    #[rstest]
    fn test_should_not_scroll_before_the_first_timestamp_of_the_application() {
        advance_time_and_refresh_timestamp(Duration::from_secs(10));
        let mut rendering_span = RenderingSpan::new(Duration::from_secs(10), Duration::default());
        advance_time_and_refresh_timestamp(Duration::from_secs(10));

        let original_span = rendering_span.to_span();
        // the span starts at the timestamp of the application start
        assert_eq!(original_span.begin(), Timestamp::app_init());

        rendering_span.scroll_left();

        let new_span = rendering_span.to_span();

        assert_eq!(original_span.begin(), new_span.begin());
        assert_eq!(original_span.end(), new_span.end());
        assert_eq!(original_span.duration(), new_span.duration());
    }

    #[rstest]
    fn test_should_not_follow_when_not_refreshed(rendering_span: RenderingSpan) {
        advance_time_and_refresh_timestamp(Duration::from_secs(10));
        assert_ne!(rendering_span.to_span().end(), Timestamp::now());
    }

    #[rstest]
    fn test_should_follow_when_refreshed(mut rendering_span: RenderingSpan) {
        advance_time_and_refresh_timestamp(Duration::from_secs(10));
        rendering_span.refresh();

        assert_eq!(rendering_span.to_span().end(), Timestamp::now());
    }

    #[rstest]
    fn test_should_not_follow_when_span_is_scrolled_left(mut rendering_span: RenderingSpan) {
        rendering_span.scroll_left();

        advance_time_and_refresh_timestamp(Duration::from_secs(10));
        rendering_span.refresh();

        assert_ne!(rendering_span.to_span().end(), Timestamp::now());
    }

    #[rstest]
    fn test_should_not_follow_when_span_is_not_scrolled_all_the_way_right(mut rendering_span: RenderingSpan) {
        rendering_span.scroll_left();
        rendering_span.scroll_left();
        rendering_span.scroll_right();

        advance_time_and_refresh_timestamp(Duration::from_secs(10));
        rendering_span.refresh();

        assert_ne!(rendering_span.to_span().end(), Timestamp::now());
    }

    #[rstest]
    fn test_should_follow_when_span_is_scrolled_back_all_the_way_right(mut rendering_span: RenderingSpan) {
        rendering_span.scroll_left();
        rendering_span.scroll_right();

        advance_time_and_refresh_timestamp(Duration::from_secs(10));
        rendering_span.refresh();

        assert_eq!(rendering_span.to_span().end(), Timestamp::now());
    }

    #[rstest]
    fn test_should_be_tolerant_with_span_in_past_within_tolerance_constraints(rendering_span: RenderingSpan) {
        let span = rendering_span.to_span();
        let other_span = Span::new(
            rendering_span.span.begin() - Duration::from_secs(10),
            rendering_span.span.begin() - rendering_span.tolerance(),
        );

        assert!(span.intersects(&other_span));
        assert!(other_span.intersects(&span));
    }

    #[rstest]
    fn test_should_not_be_tolerant_with_span_in_past_out_of_tolerance_bounds(rendering_span: RenderingSpan) {
        let span = rendering_span.to_span();
        let other_span = Span::new(
            rendering_span.span.begin() - Duration::from_secs(10),
            rendering_span.span.begin() - 2 * rendering_span.tolerance(),
        );

        assert!(!span.intersects(&other_span));
        assert!(!other_span.intersects(&span));
    }

    #[rstest]
    fn test_should_not_be_tolerant_with_span_in_future(rendering_span: RenderingSpan) {
        let span = rendering_span.to_span();

        let other_span = Span::new(
            rendering_span.span.end() + rendering_span.tolerance(),
            rendering_span.span.end() + Duration::from_secs(10),
        );

        assert!(!span.intersects(&other_span));
        assert!(!other_span.intersects(&span));
    }
}
